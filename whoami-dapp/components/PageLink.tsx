import Link from 'next/Link'

export default function PageLink({
    href,
    title,
    description,
    Icon
}: {
    href: string,
    title: string,
    description: string,
    Icon: Function
}) {
    return (
	<div className="flex flex-wrap items-center justify-around max-w-4xl mt-4 max-w-full sm:w-full">
            <Link href={href} passHref>
		<a className="p-6 mt-6 text-left border border-secondary hover:border-primary w-96 rounded-xl hover:text-primary focus:text-primary-focus">
		    <h3 className="text-2xl font-bold flex">
			<Icon className="h-5 w-5 inline mt-2 mr-2" />
			{title}
		    </h3>
		    <p className="mt-4 text-xl">
			{description}
		    </p>
		</a>
            </Link>
	</div>
    )

}
